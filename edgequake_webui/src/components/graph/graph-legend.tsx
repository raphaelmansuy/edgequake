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

export function GraphLegend({ className, collapsed = false }: GraphLegendProps) {
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
        className={`bg-background/80 backdrop-blur-sm ${className}`}
        onClick={() => setIsCollapsed(false)}
        title="Show Legend"
      >
        <Palette className="h-4 w-4" />
      </Button>
    );
  }

  return (
    <Card className={`bg-background/90 backdrop-blur-sm w-52 shadow-lg ${className}`}>
      <CardHeader className="py-2.5 px-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-xs font-semibold flex items-center gap-1.5">
            <Palette className="h-3.5 w-3.5 text-muted-foreground" />
            {t('graph.legend.title', 'Entity Types')}
          </CardTitle>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 hover:bg-muted"
            onClick={() => setIsCollapsed(true)}
            title={t('graph.collapseLegend', 'Collapse')}
          >
            <EyeOff className="h-3.5 w-3.5" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="py-0 pb-3 px-3">
        <ScrollArea className="max-h-56">
          <div className="space-y-0.5">
            {typeStats.map(({ type, count, color, label }) => (
              <button
                key={type}
                className={`w-full flex items-center gap-2.5 px-2 py-1.5 rounded-md text-xs transition-all hover:bg-muted/80 ${
                  !isVisible(type) ? 'opacity-40' : 'opacity-100'
                }`}
                onClick={() => toggleEntityType(type)}
                title={!isVisible(type) ? `${t('common.show', 'Show')} ${label}` : `${t('common.hide', 'Hide')} ${label}`}
              >
                <div
                  className="w-3.5 h-3.5 rounded-full shrink-0 ring-1 ring-black/10 shadow-sm"
                  style={{ backgroundColor: color }}
                />
                <span className="flex-1 text-left truncate font-medium">{label}</span>
                <Badge 
                  variant="secondary" 
                  className="h-5 px-1.5 text-[10px] font-semibold min-w-[28px] justify-center"
                >
                  {count}
                </Badge>
                {!isVisible(type) ? (
                  <EyeOff className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                ) : (
                  <Eye className="h-3.5 w-3.5 text-primary/60 shrink-0" />
                )}
              </button>
            ))}
          </div>
        </ScrollArea>
        
        {hiddenCount > 0 && (
          <Button
            variant="outline"
            size="sm"
            className="w-full mt-2.5 h-7 text-xs font-medium"
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
