'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useGraphStore } from '@/stores/use-graph-store';
import { Eye, EyeOff, Palette } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

// Color palette for entity types - matches graph-renderer.tsx
const TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6',
  ORGANIZATION: '#10b981',
  LOCATION: '#f59e0b',
  EVENT: '#ef4444',
  CONCEPT: '#8b5cf6',
  DOCUMENT: '#6366f1',
  DEFAULT: '#64748b',
};

interface GraphLegendProps {
  className?: string;
  collapsed?: boolean;
}

export function GraphLegend({ className, collapsed = false }: GraphLegendProps) {
  const { t } = useTranslation();
  const { nodes, visibleEntityTypes, toggleEntityType, setVisibleEntityTypes } = useGraphStore();
  const [isCollapsed, setIsCollapsed] = useState(collapsed);
  const [maxHeight, setMaxHeight] = useState(224); // Default max-h-56 = 224px
  const cardRef = useRef<HTMLDivElement>(null);

  // Dynamically calculate max height based on available viewport space
  useEffect(() => {
    const updateMaxHeight = () => {
      if (!cardRef.current) return;
      
      const cardRect = cardRef.current.getBoundingClientRect();
      const viewportHeight = window.innerHeight;
      
      // Calculate available space: viewport height - card top position - bottom padding (48px) - header/footer space (80px)
      const availableHeight = viewportHeight - cardRect.top - 48;
      
      // Content area is card height minus header (~44px) and footer button area (~40px when visible)
      // Clamp between 120px (min useful height) and 400px (max comfortable height)
      const contentMaxHeight = Math.max(120, Math.min(400, availableHeight - 100));
      
      setMaxHeight(contentMaxHeight);
    };

    updateMaxHeight();
    window.addEventListener('resize', updateMaxHeight);
    
    return () => window.removeEventListener('resize', updateMaxHeight);
  }, [isCollapsed]);

  // Calculate entity type counts from all nodes
  const typeStats = useMemo(() => {
    const stats = new Map<string, number>();
    
    nodes.forEach((node) => {
      const type = node.node_type?.toUpperCase() || 'DEFAULT';
      stats.set(type, (stats.get(type) || 0) + 1);
    });

    // Sort by count descending
    return Array.from(stats.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([type, count]) => ({
        type,
        count,
        color: TYPE_COLORS[type] || TYPE_COLORS.DEFAULT,
        label: t(`graph.nodeTypes.${type.toLowerCase()}`, type.charAt(0) + type.slice(1).toLowerCase()),
      }));
  }, [nodes, t]);

  const allTypes = useMemo(() => typeStats.map(s => s.type), [typeStats]);
  
  const isVisible = (type: string) => visibleEntityTypes.has(type);
  
  const hiddenCount = useMemo(() => {
    return allTypes.filter(type => !visibleEntityTypes.has(type)).length;
  }, [allTypes, visibleEntityTypes]);

  if (typeStats.length === 0) return null;

  if (isCollapsed) {
    return (
      <Button
        variant="outline"
        size="icon"
        className={`bg-background/80 backdrop-blur-sm ${className}`}
        onClick={() => setIsCollapsed(false)}
        aria-label={t('graph.legend.showLegend', 'Show entity type legend')}
        title={t('graph.legend.showLegend', 'Show Legend')}
      >
        <Palette className="h-4 w-4" aria-hidden="true" />
      </Button>
    );
  }

  return (
    <Card 
      ref={cardRef}
      className={`bg-background/90 backdrop-blur-sm w-52 shadow-lg max-h-[calc(100vh-120px)] flex flex-col ${className}`}
      role="region"
      aria-label={t('graph.legend.title', 'Entity Types')}
    >
      <CardHeader className="py-2.5 px-3 shrink-0">
        <div className="flex items-center justify-between">
          <CardTitle className="text-xs font-semibold flex items-center gap-1.5">
            <Palette className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
            {t('graph.legend.title', 'Entity Types')}
          </CardTitle>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 hover:bg-muted"
            onClick={() => setIsCollapsed(true)}
            aria-label={t('graph.legend.collapse', 'Collapse legend')}
            title={t('graph.collapseLegend', 'Collapse')}
          >
            <EyeOff className="h-3.5 w-3.5" aria-hidden="true" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="py-0 pb-3 px-3 flex-1 min-h-0 flex flex-col overflow-hidden">
        <ScrollArea 
          className="flex-1 min-h-0"
          style={{ maxHeight: `${maxHeight}px` }}
        >
          <div className="space-y-0.5" role="list" aria-label={t('graph.legend.typeList', 'Entity type visibility controls')}>
            {typeStats.map(({ type, count, color, label }) => (
              <button
                key={type}
                role="listitem"
                className={`w-full flex items-center gap-2.5 px-2 py-1.5 rounded-md text-xs transition-all hover:bg-muted/80 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 ${
                  !isVisible(type) ? 'opacity-40' : 'opacity-100'
                }`}
                onClick={() => toggleEntityType(type)}
                aria-pressed={isVisible(type)}
                aria-label={`${label}: ${count} ${t('graph.legend.entities', 'entities')}. ${isVisible(type) ? t('graph.legend.clickToHide', 'Click to hide') : t('graph.legend.clickToShow', 'Click to show')}`}
              >
                <div
                  className="w-3.5 h-3.5 rounded-full shrink-0 ring-1 ring-black/10 shadow-sm"
                  style={{ backgroundColor: color }}
                  aria-hidden="true"
                />
                <span className="flex-1 text-left truncate font-medium">{label}</span>
                <Badge 
                  variant="secondary" 
                  className="h-5 px-1.5 text-[10px] font-semibold min-w-[28px] justify-center"
                  aria-hidden="true"
                >
                  {count}
                </Badge>
                {!isVisible(type) ? (
                  <EyeOff className="h-3.5 w-3.5 text-muted-foreground shrink-0" aria-hidden="true" />
                ) : (
                  <Eye className="h-3.5 w-3.5 text-primary/60 shrink-0" aria-hidden="true" />
                )}
              </button>
            ))}
          </div>
        </ScrollArea>
        
        {hiddenCount > 0 && (
          <Button
            variant="outline"
            size="sm"
            className="w-full mt-2.5 h-7 text-xs font-medium shrink-0"
            onClick={() => setVisibleEntityTypes(allTypes)}
          >
            {t('graph.showAll', 'Show All')} ({hiddenCount} {t('graph.hidden', 'hidden')})
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

export default GraphLegend;
