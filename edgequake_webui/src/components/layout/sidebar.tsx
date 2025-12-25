'use client';

import { ClientOnly } from '@/components/client-only';
import { TenantWorkspaceSelector } from '@/components/shared/tenant-workspace-selector';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { useSettingsStore } from '@/stores/use-settings-store';
import { ChevronLeft, ChevronRight, FileText, Home, Menu, MessageSquare, Network, Settings, Terminal } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

const navItems = [
  { href: '/', icon: Home, labelKey: 'nav.dashboard' },
  { href: '/graph', icon: Network, labelKey: 'nav.graph' },
  { href: '/documents', icon: FileText, labelKey: 'nav.documents' },
  { href: '/query', icon: MessageSquare, labelKey: 'nav.query' },
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
          "flex h-16 items-center border-b",
          collapsed ? "justify-center px-3" : "px-5"
        )}>
          <Link 
            href="/" 
            className="flex items-center gap-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 rounded-lg"
          >
            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-primary" aria-hidden="true">
              <Network className="h-5 w-5 text-primary-foreground" />
            </div>
            {!collapsed && <span className="text-xl font-bold tracking-tight">EdgeQuake</span>}
          </Link>
        </div>

        {/* Tenant/Workspace Selector */}
        {!collapsed && (
          <div className="px-3 py-3">
            <ClientOnly fallback={null}>
              <TenantWorkspaceSelector compact={false} />
            </ClientOnly>
          </div>
        )}
        {collapsed && (
          <div className="px-2 py-3 flex justify-center">
            <ClientOnly fallback={null}>
              <TenantWorkspaceSelector compact={true} />
            </ClientOnly>
          </div>
        )}

        {/* Navigation */}
        <nav className="flex-1 space-y-1 px-3 py-2" aria-label={t('common.navigation', 'Main navigation')}>
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
                  'flex items-center rounded-xl px-3 py-3 text-sm font-medium transition-all duration-150',
                  'min-h-[44px] touch-target', // WCAG touch target
                  'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2',
                  collapsed ? 'justify-center' : 'gap-3',
                  isActive
                    ? 'bg-primary text-primary-foreground shadow-sm'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                )}
              >
                <Icon className="h-5 w-5 flex-shrink-0" aria-hidden="true" />
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
          "border-t p-4 space-y-3 transition-all duration-200",
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
                      "w-full min-h-[40px] transition-all duration-200 hover:bg-muted",
                      collapsed ? "px-0 justify-center" : "justify-start"
                    )}
                    aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
                  >
                    {collapsed ? (
                      <ChevronRight className="h-4 w-4" />
                    ) : (
                      <>
                        <ChevronLeft className="h-4 w-4 mr-2" />
                        <span className="text-sm">Collapse</span>
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
                  "flex items-center gap-3 px-1 py-1 rounded-lg transition-colors hover:bg-muted/50 cursor-default",
                  collapsed && "justify-center px-0"
                )}>
                  <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-primary/80 to-primary shrink-0">
                    <Network className="h-4 w-4 text-primary-foreground" />
                  </div>
                  {!collapsed && (
                    <div className="flex flex-col min-w-0">
                      <span className="text-sm font-semibold truncate">EdgeQuake</span>
                      <span className="text-[10px] text-muted-foreground">v0.1.0</span>
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
        <SheetContent side="left" className="w-64 p-0">
          <SidebarContent onItemClick={() => setOpen(false)} />
        </SheetContent>
      </Sheet>
    </ClientOnly>
  );
}

export default Sidebar;
