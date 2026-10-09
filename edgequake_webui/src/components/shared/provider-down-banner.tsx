'use client';

import { resolveServerRootUrl } from '@/lib/api/client';
import { AlertTriangle } from 'lucide-react';
import Link from 'next/link';
import { useEffect, useState } from 'react';

interface HealthSlice {
  components?: { llm_provider?: boolean };
}

export function ProviderDownBanner() {
  const [down, setDown] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const tick = () => {
      void fetch(resolveServerRootUrl('/health'))
        .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
        .then((h: HealthSlice) => {
          if (!cancelled) setDown(h.components?.llm_provider === false);
        })
        .catch(() => {
          if (!cancelled) setDown(false);
        });
    };
    tick();
    const id = window.setInterval(tick, 30_000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  if (!down) return null;

  return (
    <div
      role="status"
      data-testid="provider-down-banner"
      className="border-b border-amber-200 bg-amber-50 px-4 py-2 text-sm text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/80 dark:text-amber-100"
    >
      <AlertTriangle className="mr-2 inline h-4 w-4" aria-hidden />
      No LLM provider is reachable.{' '}
      <Link className="underline" href="/settings">
        Open Connections
      </Link>{' '}
      to test a local or cloud server, or run <code>edgequake doctor</code>.
    </div>
  );
}
