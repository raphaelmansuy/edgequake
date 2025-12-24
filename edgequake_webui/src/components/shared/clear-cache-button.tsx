'use client';

import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
    AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useQueryClient } from '@tanstack/react-query';
import { Eraser, Loader2 } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface ClearCacheButtonProps {
  /**
   * Whether to show as icon-only button
   */
  iconOnly?: boolean;
  /**
   * Size of the button
   */
  size?: 'default' | 'sm' | 'lg' | 'icon';
  /**
   * Variant of the button
   */
  variant?: 'default' | 'outline' | 'ghost' | 'destructive';
}

/**
 * Button component to clear application cache.
 * 
 * Currently clears:
 * - React Query cache (all queries)
 * - Local storage cache for graphs/settings if applicable
 * 
 * Future: When backend adds /api/v1/cache/clear endpoint,
 * this will also clear server-side LLM cache.
 */
export function ClearCacheButton({
  iconOnly = false,
  size = 'sm',
  variant = 'outline',
}: ClearCacheButtonProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [isClearing, setIsClearing] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);

  const handleClearCache = async () => {
    setIsClearing(true);

    try {
      // Clear React Query cache
      await queryClient.invalidateQueries();
      queryClient.clear();

      // Clear any local storage cache (optional items)
      if (typeof window !== 'undefined') {
        // Clear graph visualization cache if exists
        localStorage.removeItem('edgequake-graph-cache');
        // Clear query history cache
        localStorage.removeItem('edgequake-query-history');
        // Note: We don't clear auth tokens or tenant selection
      }

      // TODO: When backend supports it, call:
      // await api.post('/cache/clear');

      toast.success(
        t('settings.cache.cleared', 'Cache cleared'),
        {
          description: t(
            'settings.cache.clearedDesc',
            'Application cache has been cleared. Data will be refreshed from the server.'
          ),
        }
      );

      setShowConfirm(false);
    } catch (error) {
      toast.error(
        t('settings.cache.clearFailed', 'Failed to clear cache'),
        {
          description: error instanceof Error ? error.message : 'Unknown error',
        }
      );
    } finally {
      setIsClearing(false);
    }
  };

  if (iconOnly) {
    return (
      <TooltipProvider>
        <Tooltip>
          <AlertDialog open={showConfirm} onOpenChange={setShowConfirm}>
            <AlertDialogTrigger asChild>
              <TooltipTrigger asChild>
                <Button
                  variant={variant}
                  size="icon"
                  disabled={isClearing}
                >
                  {isClearing ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Eraser className="h-4 w-4" />
                  )}
                </Button>
              </TooltipTrigger>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>
                  {t('settings.cache.clearTitle', 'Clear Cache?')}
                </AlertDialogTitle>
                <AlertDialogDescription>
                  {t(
                    'settings.cache.clearDescription',
                    'This will clear all cached data in the application. You may need to wait for data to reload from the server.'
                  )}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
                <AlertDialogAction onClick={handleClearCache}>
                  {isClearing && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                  {t('settings.cache.clear', 'Clear Cache')}
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
          <TooltipContent>
            {t('settings.cache.clear', 'Clear Cache')}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return (
    <AlertDialog open={showConfirm} onOpenChange={setShowConfirm}>
      <AlertDialogTrigger asChild>
        <Button variant={variant} size={size} disabled={isClearing}>
          {isClearing ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Eraser className="mr-2 h-4 w-4" />
          )}
          {t('settings.cache.clear', 'Clear Cache')}
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t('settings.cache.clearTitle', 'Clear Cache?')}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t(
              'settings.cache.clearDescription',
              'This will clear all cached data in the application, including:'
            )}
            <ul className="list-disc list-inside mt-2 space-y-1 text-sm">
              <li>{t('settings.cache.queryCache', 'Query results cache')}</li>
              <li>{t('settings.cache.graphCache', 'Graph visualization cache')}</li>
              <li>{t('settings.cache.historyCache', 'Query history cache')}</li>
            </ul>
            <p className="mt-2">
              {t(
                'settings.cache.reloadNote',
                'Data will be refreshed from the server.'
              )}
            </p>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
          <AlertDialogAction onClick={handleClearCache}>
            {isClearing && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {t('settings.cache.clear', 'Clear Cache')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export default ClearCacheButton;
