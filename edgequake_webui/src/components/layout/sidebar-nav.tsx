'use client';

/**
 * @module SidebarNav
 * @description Sidebar contents (logo, grouped destinations, collapse toggle).
 *
 * Layouts (SPEC-168):
 * - `rail`  icon rail, labels hidden (user collapsed the sidebar)
 * - `auto`  icon rail below `lg`, full width from `lg` up (default expanded)
 * - `full`  always full width (mobile drawer)
 *
 * The same DOM serves every breakpoint, so resizing never remounts or shifts
 * the list; only CSS decides what is visible.
 */
import { KeyCombo } from '@/components/shared/kbd';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { APP_VERSION_NUMBER } from '@/lib/app-version';
import { DESTINATION_GROUPS, isActiveDestination, type Destination } from '@/lib/keyboard/destinations';
import { cn } from '@/lib/utils';
import { Network, PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useTranslation } from 'react-i18next';

export type SidebarLayout = 'rail' | 'auto' | 'full';

const LABEL_CLASS: Record<SidebarLayout, string> = {
  rail: 'hidden',
  auto: 'hidden lg:inline',
  full: 'inline',
};

const LINK_LAYOUT_CLASS: Record<SidebarLayout, string> = {
  rail: 'h-8 justify-center',
  auto: 'h-8 justify-center lg:justify-start lg:gap-2.5 lg:px-2',
  full: 'h-9 gap-2.5 px-2',
};

interface NavLinkProps {
  destination: Destination;
  layout: SidebarLayout;
  active: boolean;
  onNavigate?: () => void;
}

function NavLink({ destination, layout, active, onNavigate }: NavLinkProps) {
  const { t } = useTranslation();
  const { href, icon: Icon, labelKey, fallback, chord } = destination;
  const label = t(labelKey, fallback);

  const link = (
    <Link
      href={href}
      onClick={onNavigate}
      aria-current={active ? 'page' : undefined}
      aria-label={layout === 'full' ? undefined : label}
      className={cn(
        'group/nav relative flex items-center rounded-md text-[13.5px] transition-colors',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring',
        LINK_LAYOUT_CLASS[layout],
        active
          ? // Raised item: a hairline ring + soft shadow lift it off the warm panel.
            'bg-card font-medium text-foreground shadow-[0_1px_2px_rgb(0_0_0/0.05),0_0_0_1px_var(--border)] dark:bg-sidebar-accent dark:shadow-none'
          : 'font-[450] text-muted-foreground hover:bg-sidebar-accent hover:text-foreground',
      )}
    >
      <Icon
        className={cn(
          'h-4 w-4 shrink-0 transition-colors',
          active ? 'text-foreground/80' : 'text-muted-foreground/70 group-hover/nav:text-foreground/80',
        )}
        aria-hidden="true"
      />
      <span className={cn('truncate', LABEL_CLASS[layout])}>{label}</span>
      {layout !== 'full' && (
        <span
          aria-hidden="true"
          className="pointer-events-none absolute right-2 top-1/2 hidden -translate-y-1/2 opacity-0 transition-opacity group-hover/nav:opacity-100 group-focus-visible/nav:opacity-100 lg:group-data-[sidebar=auto]/sb:inline"
        >
          <KeyCombo keys={['G', chord.toUpperCase()]} kind="sequence" />
        </span>
      )}
    </Link>
  );

  if (layout === 'full') return link;

  return (
    <Tooltip>
      <TooltipTrigger asChild>{link}</TooltipTrigger>
      <TooltipContent
        side="right"
        sideOffset={10}
        className={cn('flex items-center gap-2', layout === 'auto' && 'lg:hidden')}
      >
        {label}
        <KeyCombo keys={['G', chord.toUpperCase()]} kind="sequence" />
      </TooltipContent>
    </Tooltip>
  );
}

const HEADING_CLASS =
  'flex h-6 items-center px-2 text-[10.5px] font-semibold uppercase tracking-[0.06em] text-muted-foreground select-none';

function GroupHeading({ layout, text }: { layout: SidebarLayout; text: string }) {
  if (layout === 'full') return <p className={HEADING_CLASS}>{text}</p>;
  return (
    <>
      <p className={cn(HEADING_CLASS, layout === 'rail' ? 'hidden' : 'hidden lg:flex')}>{text}</p>
      <div
        aria-hidden="true"
        className={cn('mx-2 my-2 border-t border-sidebar-border', layout === 'auto' && 'lg:hidden')}
      />
    </>
  );
}

interface PanelToggleProps {
  label: string;
  icon: typeof PanelLeftClose;
  onClick: () => void;
  testId: string;
  className?: string;
}

/** Icon button that sits beside the brand, with the `[` shortcut in its tooltip. */
function PanelToggle({ label, icon: Icon, onClick, testId, className }: PanelToggleProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          onClick={onClick}
          aria-keyshortcuts="["
          aria-label={label}
          data-testid={testId}
          className={cn('h-7 w-7 shrink-0 text-muted-foreground hover:bg-sidebar-accent hover:text-foreground', className)}
        >
          <Icon className="h-4 w-4" aria-hidden="true" />
        </Button>
      </TooltipTrigger>
      <TooltipContent side="right" sideOffset={8} className="flex items-center gap-2">
        {label}
        <KeyCombo keys={['[']} kind="single" />
      </TooltipContent>
    </Tooltip>
  );
}

interface SidebarNavProps {
  layout: SidebarLayout;
  onNavigate?: () => void;
  onToggle?: () => void;
  /** Opens the full-width drawer; shown below `lg` where the rail cannot widen. */
  onOpenDrawer?: () => void;
}

export function SidebarNav({ layout, onNavigate, onToggle, onOpenDrawer }: SidebarNavProps) {
  const pathname = usePathname();
  const { t } = useTranslation();
  const collapsed = layout === 'rail';

  return (
    <TooltipProvider delayDuration={300}>
      <div className="group/sb flex h-full flex-col" data-sidebar={layout}>
        <div
          className={cn(
            'flex shrink-0 gap-1 p-2',
            layout === 'full'
              ? 'items-center justify-between'
              : cn('flex-col items-center', layout === 'auto' && 'lg:flex-row lg:justify-between'),
          )}
        >
          <Link
            href="/"
            aria-label="EdgeQuake"
            onClick={onNavigate}
            className="flex min-w-0 items-center gap-2 rounded-md p-1 hover:bg-sidebar-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <span className="flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-md bg-primary" aria-hidden="true">
              <Network className="h-3 w-3 text-primary-foreground" />
            </span>
            <span className={cn('truncate text-[13.5px] font-semibold tracking-tight', LABEL_CLASS[layout])}>EdgeQuake</span>
          </Link>
          {onToggle && (
            <>
              {/* ≥ lg: persisted collapse / expand */}
              <PanelToggle
                label={collapsed ? t('nav.expandSidebar', 'Expand sidebar') : t('nav.collapseSidebar', 'Collapse sidebar')}
                icon={collapsed ? PanelLeftOpen : PanelLeftClose}
                onClick={onToggle}
                testId="sidebar-toggle"
                className="max-lg:hidden"
              />
              {/* < lg: the rail cannot widen, so expanding opens the drawer */}
              {onOpenDrawer && (
                <PanelToggle
                  label={t('nav.expandSidebar', 'Expand sidebar')}
                  icon={PanelLeftOpen}
                  onClick={onOpenDrawer}
                  testId="sidebar-open-drawer"
                  className="lg:hidden"
                />
              )}
            </>
          )}
        </div>

        <nav
          className="flex-1 overflow-y-auto px-2 pb-3 pt-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
          aria-label={t('common.navigation', 'Main navigation')}
        >
          {DESTINATION_GROUPS.map((group, index) => (
            <div key={group.id} className={cn('space-y-px', index > 0 && (layout === 'full' ? 'mt-3.5' : 'mt-2 lg:mt-3.5'))}>
              {group.label && (
                <GroupHeading layout={layout} text={group.labelKey ? t(group.labelKey, group.label) : group.label} />
              )}
              {group.items.map((destination) => (
                <NavLink
                  key={destination.href}
                  destination={destination}
                  layout={layout}
                  active={isActiveDestination(pathname, destination.href)}
                  onNavigate={onNavigate}
                />
              ))}
            </div>
          ))}
        </nav>

        <p
          className={cn(
            'shrink-0 px-4 py-2.5 text-[10.5px] text-muted-foreground',
            layout === 'full' ? 'block' : 'hidden',
            layout === 'auto' && 'lg:block',
          )}
        >
          {t('common.uiVersion', { version: APP_VERSION_NUMBER })}
        </p>
      </div>
    </TooltipProvider>
  );
}
