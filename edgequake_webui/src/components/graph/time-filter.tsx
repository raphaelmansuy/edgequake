'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { useGraphStore } from '@/stores/use-graph-store';
import { Calendar, X } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface TimeFilterProps {
  className?: string;
  collapsed?: boolean;
}

export function TimeFilter({ className, collapsed = false }: TimeFilterProps) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(collapsed);
  
  const {
    nodes,
    timeFilterEnabled,
    timeFilterStart,
    timeFilterEnd,
    setTimeFilterEnabled,
    setTimeFilterRange,
    clearTimeFilter,
  } = useGraphStore();

  // Calculate date range from nodes
  const dateRange = useMemo(() => {
    const dates = nodes
      .filter((n) => n.created_at)
      .map((n) => new Date(n.created_at!).getTime());
    
    if (dates.length === 0) return null;
    
    return {
      min: new Date(Math.min(...dates)),
      max: new Date(Math.max(...dates)),
    };
  }, [nodes]);

  // Count nodes in current filter range
  const filteredCount = useMemo(() => {
    if (!timeFilterEnabled) return nodes.length;
    
    return nodes.filter((node) => {
      if (!node.created_at) return true;
      const nodeDate = new Date(node.created_at);
      if (timeFilterStart && nodeDate < timeFilterStart) return false;
      if (timeFilterEnd && nodeDate > timeFilterEnd) return false;
      return true;
    }).length;
  }, [nodes, timeFilterEnabled, timeFilterStart, timeFilterEnd]);

  const handleStartDateChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const date = e.target.value ? new Date(e.target.value) : null;
      setTimeFilterRange(date, timeFilterEnd);
    },
    [timeFilterEnd, setTimeFilterRange]
  );

  const handleEndDateChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const date = e.target.value ? new Date(e.target.value) : null;
      setTimeFilterRange(timeFilterStart, date);
    },
    [timeFilterStart, setTimeFilterRange]
  );

  const formatDateForInput = (date: Date | null): string => {
    if (!date) return '';
    return date.toISOString().split('T')[0];
  };

  // Don't show if no nodes have dates
  if (!dateRange) return null;

  if (isCollapsed) {
    return (
      <Button
        variant="outline"
        size="icon"
        className={`bg-background/80 backdrop-blur-sm ${className}`}
        onClick={() => setIsCollapsed(false)}
        aria-label={t('graph.timeFilter.show', 'Show time filter')}
        title={t('graph.timeFilter.show', 'Time Filter')}
      >
        <Calendar className="h-4 w-4" aria-hidden="true" />
        {timeFilterEnabled && (
          <span className="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-primary" />
        )}
      </Button>
    );
  }

  return (
    <Card
      className={`bg-background/80 backdrop-blur-sm shadow-lg w-64 ${className}`}
    >
      <CardHeader className="p-3 pb-1 flex flex-row items-center justify-between">
        <CardTitle className="text-sm font-medium flex items-center gap-1.5">
          <Calendar className="h-4 w-4" aria-hidden="true" />
          {t('graph.timeFilter.title', 'Time Filter')}
        </CardTitle>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={() => setIsCollapsed(true)}
          aria-label={t('graph.timeFilter.collapse', 'Collapse time filter')}
        >
          <X className="h-3.5 w-3.5" />
        </Button>
      </CardHeader>

      <CardContent className="p-3 pt-1 space-y-3">
        {/* Enable/Disable Toggle */}
        <div className="flex items-center justify-between">
          <Label htmlFor="time-filter-toggle" className="text-xs text-muted-foreground">
            {t('graph.timeFilter.enable', 'Enable filtering')}
          </Label>
          <Switch
            id="time-filter-toggle"
            checked={timeFilterEnabled}
            onCheckedChange={setTimeFilterEnabled}
          />
        </div>

        {/* Date Range Info */}
        <div className="text-xs text-muted-foreground">
          {t('graph.timeFilter.range', 'Data range')}: {dateRange.min.toLocaleDateString()} - {dateRange.max.toLocaleDateString()}
        </div>

        {/* Start Date */}
        <div className="space-y-1">
          <Label htmlFor="start-date" className="text-xs">
            {t('graph.timeFilter.from', 'From')}
          </Label>
          <input
            id="start-date"
            type="date"
            className="flex h-8 w-full rounded-md border border-input bg-background px-2 py-1 text-xs ring-offset-background file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
            value={formatDateForInput(timeFilterStart)}
            min={formatDateForInput(dateRange.min)}
            max={formatDateForInput(timeFilterEnd || dateRange.max)}
            onChange={handleStartDateChange}
            disabled={!timeFilterEnabled}
          />
        </div>

        {/* End Date */}
        <div className="space-y-1">
          <Label htmlFor="end-date" className="text-xs">
            {t('graph.timeFilter.to', 'To')}
          </Label>
          <input
            id="end-date"
            type="date"
            className="flex h-8 w-full rounded-md border border-input bg-background px-2 py-1 text-xs ring-offset-background file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
            value={formatDateForInput(timeFilterEnd)}
            min={formatDateForInput(timeFilterStart || dateRange.min)}
            max={formatDateForInput(dateRange.max)}
            onChange={handleEndDateChange}
            disabled={!timeFilterEnabled}
          />
        </div>

        {/* Results Count */}
        <div className="flex items-center justify-between pt-1 border-t">
          <span className="text-xs text-muted-foreground">
            {t('graph.timeFilter.showing', 'Showing')}
          </span>
          <span className="text-xs font-medium">
            {filteredCount} / {nodes.length} {t('graph.timeFilter.nodes', 'nodes')}
          </span>
        </div>

        {/* Clear Button */}
        {timeFilterEnabled && (
          <Button
            variant="outline"
            size="sm"
            className="w-full text-xs h-7"
            onClick={clearTimeFilter}
          >
            {t('graph.timeFilter.clear', 'Clear Filter')}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}
