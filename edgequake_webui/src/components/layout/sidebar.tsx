'use client';

import { ClientOnly } from '@/components/client-only';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet';
import { cn } from '@/lib/utils';
import { FileText, Menu, MessageSquare, Network, Settings, Terminal } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

const navItems = [
  { href: '/graph', icon: Network, labelKey: 'nav.graph' },
  { href: '/documents', icon: FileText, labelKey: 'nav.documents' },
  { href: '/query', icon: MessageSquare, labelKey: 'nav.query' },
  { href: '/api-explorer', icon: Terminal, labelKey: 'nav.apiExplorer' },
  { href: '/settings', icon: Settings, labelKey: 'nav.settings' },
];

function SidebarContent({ onItemClick }: { onItemClick?: () => void }) {
  const pathname = usePathname();
  const { t } = useTranslation();

  return (
    <div className="flex h-full flex-col">
      {/* Logo */}
      <div className="flex h-16 items-center border-b px-4">
        <Link href="/graph" className="flex items-center gap-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 rounded-lg">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary" aria-hidden="true">
            <Network className="h-5 w-5 text-primary-foreground" />
          </div>
          <span className="text-xl font-bold">EdgeQuake</span>
        </Link>
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 p-2" aria-label={t('common.navigation', 'Main navigation')}>
        {navItems.map(({ href, icon: Icon, labelKey }) => {
          const isActive = pathname === href || pathname.startsWith(href + '/');
          
          return (
            <Link
              key={href}
              href={href}
              onClick={onItemClick}
              aria-current={isActive ? 'page' : undefined}
              className={cn(
                'flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2',
                isActive
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'
              )}
            >
              <Icon className="h-5 w-5" aria-hidden="true" />
              <span>{t(labelKey)}</span>
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="border-t p-4">
        <div className="text-xs text-muted-foreground">
          <p>EdgeQuake v0.1.0</p>
          <p>{t('common.platform')}</p>
        </div>
      </div>
    </div>
  );
}

export function Sidebar() {
  return (
    <aside className="hidden w-64 border-r bg-card md:block" aria-label="Sidebar navigation">
      <SidebarContent />
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
