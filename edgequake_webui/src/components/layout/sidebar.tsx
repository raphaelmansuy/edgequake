/**
 * @module Sidebar
 * @description Application sidebar shell: desktop rail/expanded sidebar and the
 * mobile drawer. Destinations come from `lib/keyboard/destinations.ts`; the
 * list itself lives in `sidebar-nav.tsx`.
 *
 * @implements FEAT0602 - Responsive navigation sidebar
 * @implements FEAT0609 - Collapsible sidebar with state persistence
 * @implements FEAT0610 - Mobile-optimized drawer navigation
 *
 * @enforces BR0606 - Sidebar state persists across sessions
 * @enforces BR0607 - Active route highlighted in navigation
 *
 * @see {@link docs/features.md} FEAT0602, FEAT0609
 */
'use client';

import { ClientOnly } from '@/components/client-only';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { NAV_DRAWER_EVENT } from '@/lib/keyboard/nav-drawer';
import { createReturnFocus } from '@/lib/keyboard/return-focus';
import { cn } from '@/lib/utils';
import { useSettingsStore } from '@/stores/use-settings-store';
import { Menu } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { HeaderTenantSelector } from './header-tenant-selector';
import { SidebarNav } from './sidebar-nav';

/** Full-width navigation drawer: phones (hamburger) and the rail below `lg`. */
function NavDrawer({
  open,
  onOpenChange,
  onCloseAutoFocus,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCloseAutoFocus: (event: Event) => void;
}) {
  const { t } = useTranslation();
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="left" size="sm" className="flex w-[252px] flex-col bg-sidebar p-0" onCloseAutoFocus={onCloseAutoFocus}>
        <SheetHeader className="sr-only">
          <SheetTitle>{t('common.navigation', 'Main navigation')}</SheetTitle>
          <SheetDescription>EdgeQuake</SheetDescription>
        </SheetHeader>
        <div className="border-b border-sidebar-border p-3">
          <HeaderTenantSelector className="w-full" />
        </div>
        <div className="flex-1 overflow-hidden">
          <SidebarNav layout="full" onNavigate={() => onOpenChange(false)} />
        </div>
      </SheetContent>
    </Sheet>
  );
}

export function Sidebar() {
  const sidebarCollapsed = useSettingsStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useSettingsStore((s) => s.toggleSidebar);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [returnFocus] = useState(createReturnFocus);
  const openDrawer = () => {
    returnFocus.remember();
    setDrawerOpen(true);
  };

  // `[` below lg asks for the drawer through the shared navigation rule.
  useEffect(() => {
    const open = () => {
      returnFocus.remember();
      setDrawerOpen((v) => !v);
    };
    window.addEventListener(NAV_DRAWER_EVENT, open);
    return () => window.removeEventListener(NAV_DRAWER_EVENT, open);
  }, [returnFocus]);

  return (
    <>
      <aside
        className={cn(
          'hidden shrink-0 border-r border-sidebar-border bg-sidebar md:block',
          // Below `lg` the sidebar is always a rail so content keeps its width.
          sidebarCollapsed ? 'w-[52px]' : 'w-[52px] lg:w-[252px]',
        )}
        aria-label="Sidebar navigation"
        data-testid="sidebar"
        data-collapsed={sidebarCollapsed}
      >
        <SidebarNav
          layout={sidebarCollapsed ? 'rail' : 'auto'}
          onToggle={toggleSidebar}
          onOpenDrawer={openDrawer}
        />
      </aside>
      <ClientOnly fallback={null}>
        <NavDrawer open={drawerOpen} onOpenChange={setDrawerOpen} onCloseAutoFocus={returnFocus.restore} />
      </ClientOnly>
    </>
  );
}

export function MobileSidebar() {
  const [open, setOpen] = useState(false);
  const [returnFocus] = useState(createReturnFocus);
  const { t } = useTranslation();
  const label = t('nav.openMenu', 'Open navigation menu');
  const trigger = (
    <Button
      variant="ghost"
      size="icon"
      className="h-8 w-8 md:hidden"
      aria-label={label}
      onClick={() => {
        returnFocus.remember();
        setOpen(true);
      }}
    >
      <Menu className="h-5 w-5" aria-hidden="true" />
    </Button>
  );

  return (
    <>
      {trigger}
      <ClientOnly fallback={null}>
        <NavDrawer open={open} onOpenChange={setOpen} onCloseAutoFocus={returnFocus.restore} />
      </ClientOnly>
    </>
  );
}

export default Sidebar;
