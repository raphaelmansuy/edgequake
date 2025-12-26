'use client';

import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';
import { ChevronRight, FileText, Home, MessageSquare, Network, Settings, Terminal } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import React from 'react';

interface PathConfig {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  description?: string;
}

const pathConfig: Record<string, PathConfig> = {
  '': { label: 'Dashboard', icon: Home, description: 'Overview and statistics' },
  'graph': { label: 'Knowledge Graph', icon: Network, description: 'Visualize entities and relationships' },
  'documents': { label: 'Documents', icon: FileText, description: 'Manage your documents' },
  'query': { label: 'Query', icon: MessageSquare, description: 'Ask questions' },
  'api-explorer': { label: 'API Explorer', icon: Terminal, description: 'Test API endpoints' },
  'settings': { label: 'Settings', icon: Settings, description: 'Configure preferences' },
};

interface DynamicBreadcrumbProps {
  /** Additional custom segments to append */
  customSegments?: Array<{ label: string; href?: string }>;
}

export function DynamicBreadcrumb({ customSegments }: DynamicBreadcrumbProps) {
  const pathname = usePathname();
  
  // Parse the path into segments
  const segments = pathname.split('/').filter(Boolean);
  
  // Build breadcrumb items
  const items: Array<{ label: string; href: string; icon?: React.ComponentType<{ className?: string }> }> = [
    { label: 'EdgeQuake', href: '/', icon: Home },
  ];
  
  let currentPath = '';
  segments.forEach((segment) => {
    currentPath += `/${segment}`;
    const config = pathConfig[segment];
    if (config) {
      items.push({
        label: config.label,
        href: currentPath,
        icon: config.icon,
      });
    } else {
      // Handle dynamic segments (e.g., document IDs, entity IDs)
      items.push({
        label: decodeURIComponent(segment).slice(0, 12) + (segment.length > 12 ? '...' : ''),
        href: currentPath,
      });
    }
  });
  
  // Add custom segments if provided
  if (customSegments) {
    customSegments.forEach((seg) => {
      items.push({
        label: seg.label,
        href: seg.href || '#',
      });
    });
  }
  
  // Don't show breadcrumbs on the root page
  if (items.length <= 1) {
    return null;
  }

  return (
    <Breadcrumb>
      <BreadcrumbList>
        {items.map((item, index) => {
          const isLast = index === items.length - 1;
          const Icon = item.icon;
          
          return (
            <React.Fragment key={item.href}>
              <BreadcrumbItem>
                {!isLast ? (
                  <BreadcrumbLink asChild>
                    <Link
                      href={item.href}
                      className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                    >
                      {Icon && <Icon className="h-3 w-3" />}
                      <span>{item.label}</span>
                    </Link>
                  </BreadcrumbLink>
                ) : (
                  <BreadcrumbPage className="flex items-center gap-1 text-xs font-medium">
                    {Icon && <Icon className="h-3 w-3" />}
                    <span>{item.label}</span>
                  </BreadcrumbPage>
                )}
              </BreadcrumbItem>
              {!isLast && (
                <BreadcrumbSeparator>
                  <ChevronRight className="h-3 w-3" />
                </BreadcrumbSeparator>
              )}
            </React.Fragment>
          );
        })}
      </BreadcrumbList>
    </Breadcrumb>
  );
}

export default DynamicBreadcrumb;
