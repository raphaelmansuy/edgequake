'use client';

import { buildHeaders } from '@/lib/api/client';
import { useEffect, useState } from 'react';

export type AuthedImage =
  | { status: 'loading' }
  | { status: 'ready'; url: string }
  | { status: 'failed' };

type Settled = { src: string; image: Exclude<AuthedImage, { status: 'loading' }> };

/**
 * An `<img src>` cannot send the tenant, workspace or bearer headers, so a protected image is
 * fetched with them and shown from a blob URL. The URL is released when `src` changes or the
 * component goes away.
 */
export function useAuthedImageUrl(src: string | null): AuthedImage {
  const [settled, setSettled] = useState<Settled | null>(null);

  useEffect(() => {
    if (!src) return;
    let objectUrl: string | null = null;
    const abort = new AbortController();
    (async () => {
      const headers = buildHeaders();
      headers.delete('Content-Type');
      const res = await fetch(src, { headers, signal: abort.signal });
      if (!res.ok) throw new Error(String(res.status));
      objectUrl = URL.createObjectURL(await res.blob());
      setSettled({ src, image: { status: 'ready', url: objectUrl } });
    })().catch(() => {
      if (!abort.signal.aborted) setSettled({ src, image: { status: 'failed' } });
    });
    return () => {
      abort.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [src]);

  if (!src) return { status: 'failed' };
  // A result for an earlier `src` is stale: show loading until this one settles.
  return settled?.src === src ? settled.image : { status: 'loading' };
}
