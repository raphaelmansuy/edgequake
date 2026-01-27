/**
 * @module Sidebar
 * @description Application sidebar navigation component.
 * Supports collapsible mode, mobile drawer, and workspace selection.
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
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from '@/components/ui/sheet';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { useSettingsStore } from '@/stores/use-settings-store';
import { Activity, ChevronLeft, ChevronRight, DollarSign, FileText, FolderKanban, Home, Menu, MessageSquare, Network, Settings, Terminal } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { HeaderTenantSelector } from './header-tenant-selector';

const navItems = [
  { href: '/', icon: Home, labelKey: 'nav.dashboard' },
  { href: '/graph', icon: Network, labelKey: 'nav.graph' },
  { href: '/documents', icon: FileText, labelKey: 'nav.documents' },
  { href: '/pipeline', icon: Activity, labelKey: 'nav.pipeline' },
  { href: '/query', icon: MessageSquare, labelKey: 'nav.query' },
  { href: '/workspace', icon: FolderKanban, labelKey: 'nav.workspace' },
  { href: '/costs', icon: DollarSign, labelKey: 'nav.costs' },
  { href: '/api-explorer', icon: Terminal, labelKey: 'nav.apiExplorer' },
  { href: '/settings', icon: Settings, labelKey: 'nav.settings' },
];

function SidebarContent({ 
  onItemClick, 
  collapsed = false,
  showToggle = false,
  onToggle,
}: { 
  onItemClick?: () => void;
  collapsed?: boolean;
  showToggle?: boolean;
  onToggle?: () => void;
}) {
  const pathname = usePathname();
  const { t } = useTranslation();

  return (
    <TooltipProvider delayDuration={0}>
      <div className="flex h-full flex-col">
        {/* Logo */}
        <div className={cn(
          "flex h-12 items-center border-b shrink-0",
          collapsed ? "justify-center px-2" : "px-4"
        )}>
          <Link 
            href="/" 
            className="flex items-center gap-2.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 rounded-lg"
          >
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary" aria-hidden="true">
              <Network className="h-4 w-4 text-primary-foreground" />
            </div>
            {!collapsed && <span className="text-lg font-bold tracking-tight">EdgeQuake</span>}
          </Link>
        </div>

        {/* Navigation */}
        <nav className="flex-1 space-y-0.5 px-2 py-3" aria-label={t('common.navigation', 'Main navigation')}>
          {navItems.map(({ href, icon: Icon, labelKey }) => {
            // Handle home page "/" specially to avoid matching all paths
            const isActive = href === '/' 
              ? pathname === '/' 
              : pathname === href || pathname.startsWith(href + '/');
            
            const linkContent = (
              <Link
                key={href}
                href={href}
                onClick={onItemClick}
                aria-current={isActive ? 'page' : undefined}
                className={cn(
                  'flex items-center rounded-lg px-3 py-2.5 text-sm font-medium transition-all duration-150',
                  'min-h-[40px]', // Slightly smaller touch target but still accessible
                  'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2',
                  collapsed ? 'justify-center' : 'gap-2.5',
                  isActive
                    ? 'bg-primary text-primary-foreground shadow-sm'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                )}
              >
                <Icon className="h-4 w-4 flex-shrink-0" aria-hidden="true" />
                {!collapsed && <span>{t(labelKey)}</span>}
              </Link>
            );

            if (collapsed) {
              return (
                <Tooltip key={href}>
                  <TooltipTrigger asChild>
                    {linkContent}
                  </TooltipTrigger>
                  <TooltipContent side="right" sideOffset={12}>
                    {t(labelKey)}
                  </TooltipContent>
                </Tooltip>
              );
            }

            return linkContent;
          })}
        </nav>

        {/* Footer */}
        <div className={cn(
          "border-t p-3 space-y-2 transition-all duration-200",
          collapsed && "p-2"
        )}>
          {showToggle && (
            <TooltipProvider delayDuration={0}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={onToggle}
                    className={cn(
                      "w-full min-h-[36px] transition-all duration-200 hover:bg-muted",
                      collapsed ? "px-0 justify-center" : "justify-start"
                    )}
                    aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
                  >
                    {collapsed ? (
                      <ChevronRight className="h-4 w-4" />
                    ) : (
                      <>
                        <ChevronLeft className="h-4 w-4 mr-2" />
                        <span className="text-xs">Collapse</span>
                      </>
                    )}
                  </Button>
                </TooltipTrigger>
                {collapsed && (
                  <TooltipContent side="right" sideOffset={12}>
                    Expand sidebar
                  </TooltipContent>
                )}
              </Tooltip>
            </TooltipProvider>
          )}
          
          {/* App Info */}
          <TooltipProvider delayDuration={0}>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className={cn(
                  "flex items-center gap-2.5 px-1 py-1 rounded-lg transition-colors hover:bg-muted/50 cursor-default",
                  collapsed && "justify-center px-0"
                )}>
                  <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-gradient-to-br from-primary/80 to-primary shrink-0">
                    <Network className="h-3.5 w-3.5 text-primary-foreground" />
                  </div>
                  {!collapsed && (
                    <div className="flex flex-col min-w-0">
                      <span className="text-xs font-semibold truncate">EdgeQuake</span>
                      <span className="text-[9px] text-muted-foreground">v0.1.0</span>
                    </div>
                  )}
                </div>
              </TooltipTrigger>
              {collapsed && (
                <TooltipContent side="right" sideOffset={12}>
                  <p className="font-semibold">EdgeQuake v0.1.0</p>
                  <p className="text-xs text-muted-foreground">{t('common.platform')}</p>
                </TooltipContent>
              )}
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>
    </TooltipProvider>
  );
}

export function Sidebar() {
  const { sidebarCollapsed, toggleSidebar } = useSettingsStore();
  
  return (
    <aside 
      className={cn(
        "hidden border-r bg-card md:block transition-all duration-300",
        sidebarCollapsed ? "w-16" : "w-64"
      )} 
      aria-label="Sidebar navigation"
    >
      <SidebarContent 
        collapsed={sidebarCollapsed}
        showToggle={true}
        onToggle={toggleSidebar}
      />
    </aside>
  );
}

export function MobileSidebar() {
  const [open, setOpen] = useState(false);

  return (
    <ClientOnly fallback={<Button variant="ghost" size="icon" className="md:hidden"><Menu className="h-5 w-5" /></Button>}>
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetTrigger asChild>
          <Button variant="ghost" size="icon" className="md:hidden">
            <Menu className="h-5 w-5" />
            <span className="sr-only">Toggle menu</span>
          </Button>
        </SheetTrigger>
        <SheetContent side="left" className="w-64 p-0 flex flex-col">
          <SheetHeader className="sr-only">
            <SheetTitle>Navigation Menu</SheetTitle>
            <SheetDescription>Main navigation for EdgeQuake application</SheetDescription>
          </SheetHeader>
          {/* Tenant Selector for Mobile */}
          <div className="border-b p-3">
            <HeaderTenantSelector className="w-full" />
          </div>
          <div className="flex-1 overflow-hidden">
            <SidebarContent onItemClick={() => setOpen(false)} />
          </div>
        </SheetContent>
      </Sheet>
    </ClientOnly>
  );
}

export default Sidebar;
