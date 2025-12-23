'use client';

import { ClientOnly } from '@/components/client-only';
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
          collapsed ? "justify-center px-2" : "px-4"
        )}>
          <Link 
            href="/" 
            className="flex items-center gap-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 rounded-lg"
          >
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary" aria-hidden="true">
              <Network className="h-5 w-5 text-primary-foreground" />
            </div>
            {!collapsed && <span className="text-xl font-bold">EdgeQuake</span>}
          </Link>
        </div>

        {/* Navigation */}
        <nav className="flex-1 space-y-1 p-2" aria-label={t('common.navigation', 'Main navigation')}>
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
                  'flex items-center rounded-lg px-3 py-2.5 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2',
                  collapsed ? 'justify-center' : 'gap-3',
                  isActive
                    ? 'bg-primary text-primary-foreground'
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
                  <TooltipContent side="right" sideOffset={10}>
                    {t(labelKey)}
                  </TooltipContent>
                </Tooltip>
              );
            }

            return linkContent;
          })}
        </nav>

        {/* Footer */}
        <div className={cn("border-t p-4", collapsed && "p-2")}>
          {showToggle && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onToggle}
              className={cn(
                "w-full mb-2",
                collapsed && "px-0"
              )}
              aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
              {collapsed ? (
                <ChevronRight className="h-4 w-4" />
              ) : (
                <>
                  <ChevronLeft className="h-4 w-4 mr-2" />
                  <span>Collapse</span>
                </>
              )}
            </Button>
          )}
          {!collapsed && (
            <div className="text-xs text-muted-foreground">
              <p>EdgeQuake v0.1.0</p>
              <p>{t('common.platform')}</p>
            </div>
          )}
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
