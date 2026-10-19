'use client';

/**
 * SPEC-168 — settings section index.
 *
 * The page is one long scroll; this sticky chip bar turns it into a map:
 * every chip is a real link (`#section`) so it works with the keyboard, and the
 * current section is tracked with an IntersectionObserver rooted on the page's
 * scroll viewport (`aria-current="location"`).
 */
import { cn } from '@/lib/utils';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface SettingsSectionDef {
  id: string;
  labelKey: string;
  fallback: string;
}

export const SETTINGS_SECTIONS: readonly SettingsSectionDef[] = [
  { id: 'general', labelKey: 'settings.sections.general', fallback: 'General' },
  { id: 'models', labelKey: 'settings.sections.models', fallback: 'Models & providers' },
  { id: 'access', labelKey: 'settings.sections.access', fallback: 'Access' },
  { id: 'observability', labelKey: 'settings.sections.observability', fallback: 'Observability' },
  { id: 'data', labelKey: 'settings.sections.data', fallback: 'Data' },
];

function useActiveSection(ids: readonly string[]): string {
  const [active, setActive] = useState(ids[0] ?? '');

  useEffect(() => {
    const sections = ids
      .map((id) => document.getElementById(`settings-${id}`))
      .filter((el): el is HTMLElement => el !== null);
    if (sections.length === 0) return;
    const root = sections[0].closest<HTMLElement>('[data-radix-scroll-area-viewport]');
    const visible = new Map<string, number>();

    const observer = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          const id = e.target.id.replace('settings-', '');
          if (e.isIntersecting) visible.set(id, e.boundingClientRect.top);
          else visible.delete(id);
        }
        const first = ids.find((id) => visible.has(id));
        if (first) setActive(first);
      },
      // The band just under the sticky bar decides which section is "current".
      { root, rootMargin: '-72px 0px -60% 0px', threshold: 0 },
    );
    sections.forEach((s) => observer.observe(s));
    return () => observer.disconnect();
  }, [ids]);

  return active;
}

export function SettingsNav() {
  const { t } = useTranslation();
  const active = useActiveSection(SETTINGS_SECTIONS.map((s) => s.id));

  return (
    <nav
      aria-label={t('settings.sections.label', 'Settings sections')}
      className="sticky top-0 z-10 -mx-4 border-b bg-background/90 px-4 backdrop-blur sm:-mx-6 sm:px-6"
      data-testid="settings-nav"
    >
      <ul className="flex gap-1 overflow-x-auto py-2 [scrollbar-width:none]">
        {SETTINGS_SECTIONS.map((s) => {
          const current = active === s.id;
          return (
            <li key={s.id} className="shrink-0">
              <a
                href={`#settings-${s.id}`}
                aria-current={current ? 'location' : undefined}
                className={cn(
                  'inline-flex h-8 items-center rounded-md px-3 text-sm font-medium transition-colors',
                  'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                  current
                    ? 'bg-muted text-foreground'
                    : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
                )}
              >
                {t(s.labelKey, s.fallback)}
              </a>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}

/** Anchored group of cards with an accessible heading. */
export function SettingsSection({ id, children }: { id: string; children: React.ReactNode }) {
  const { t } = useTranslation();
  const def = SETTINGS_SECTIONS.find((s) => s.id === id);
  const headingId = `settings-${id}-title`;
  return (
    <section id={`settings-${id}`} aria-labelledby={headingId} className="scroll-mt-14 space-y-page">
      <h2 id={headingId} className="pt-2 text-sm font-semibold uppercase tracking-wider text-muted-foreground">
        {def ? t(def.labelKey, def.fallback) : id}
      </h2>
      {children}
    </section>
  );
}
