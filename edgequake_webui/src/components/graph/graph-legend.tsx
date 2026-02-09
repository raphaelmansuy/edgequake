'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useGraphStore } from '@/stores/use-graph-store';
import { Eye, EyeOff, Palette } from 'lucide-react';
import { useMemo, useState } from 'react';
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

export function GraphLegend({ className, collapsed = true }: GraphLegendProps) {
  const { t } = useTranslation();
  const { nodes, visibleEntityTypes, toggleEntityType, setVisibleEntityTypes } = useGraphStore();
  const [isCollapsed, setIsCollapsed] = useState(collapsed);

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
        className={`bg-background/90 backdrop-blur-sm shadow-md hover:shadow-lg transition-shadow ${className}`}
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
      className={`bg-background/95 backdrop-blur-sm w-80 shadow-xl border-border/50 flex flex-col max-h-[calc(100vh-8rem)] ${className}`}
      role="region"
      aria-label={t('graph.legend.title', 'Entity Types')}
    >
      <CardHeader className="py-3 px-4 shrink-0 border-b">
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="text-sm font-semibold flex items-center gap-2.5">
            <Palette className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
            <span>{t('graph.legend.title', 'Entity Types')}</span>
          </CardTitle>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 hover:bg-muted -mr-1 shrink-0"
            onClick={() => setIsCollapsed(true)}
            aria-label={t('graph.legend.collapse', 'Collapse legend')}
            title={t('graph.collapseLegend', 'Collapse')}
          >
            <EyeOff className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="p-0 flex-1 min-h-0 flex flex-col overflow-hidden">
        <ScrollArea className="flex-1" showShadows>
          <div className="p-3 space-y-1" role="list" aria-label={t('graph.legend.typeList', 'Entity type visibility controls')}>
            {typeStats.map(({ type, count, color, label }) => (
              <button
                key={type}
                role="listitem"
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-all hover:bg-muted/70 active:bg-muted focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 ${
                  !isVisible(type) ? 'opacity-40' : 'opacity-100'
                }`}
                onClick={() => toggleEntityType(type)}
                aria-pressed={isVisible(type)}
                aria-label={`${label}: ${count} ${t('graph.legend.entities', 'entities')}. ${isVisible(type) ? t('graph.legend.clickToHide', 'Click to hide') : t('graph.legend.clickToShow', 'Click to show')}`}
              >
                <div
                  className="w-3.5 h-3.5 rounded-full shrink-0 ring-2 ring-background shadow-sm"
                  style={{ backgroundColor: color }}
                  aria-hidden="true"
                />
                <span className="flex-1 text-left font-medium leading-tight truncate min-w-0">{label}</span>
                <Badge
                  variant="secondary"
                  className="h-5 px-2 text-[10px] font-semibold tabular-nums shrink-0"
                  aria-hidden="true"
                >
                  {count}
                </Badge>
                {!isVisible(type) ? (
                  <EyeOff className="h-4 w-4 text-muted-foreground shrink-0" aria-hidden="true" />
                ) : (
                  <Eye className="h-4 w-4 text-primary/70 shrink-0" aria-hidden="true" />
                )}
              </button>
            ))}
          </div>
        </ScrollArea>

        {hiddenCount > 0 && (
          <div className="p-3 pt-0 shrink-0">
            <Button
              variant="outline"
              size="sm"
              className="w-full h-9 text-xs font-medium"
              onClick={() => setVisibleEntityTypes(allTypes)}
            >
              {t('graph.showAll', 'Show All')} ({hiddenCount} {t('graph.hidden', 'hidden')})
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export default GraphLegend;
