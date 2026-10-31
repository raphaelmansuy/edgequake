'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

export interface QuerySearchImage {
  file: File;
  /** Object URL for the thumbnail. Revoked when the image is cleared or replaced. */
  preview: string;
}

/** The formats the server reads as a query image. */
export const SEARCH_IMAGE_MIME = ['image/png', 'image/jpeg', 'image/webp'] as const;
const ACCEPTED: readonly string[] = SEARCH_IMAGE_MIME;

/**
 * SPEC-180 — the one image a user searches by. It is held in memory, sent with the question,
 * and never stored as a document. A new pick replaces the old one.
 */
export function useQuerySearchImage() {
  const [image, setImage] = useState<QuerySearchImage | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const clear = useCallback(() => setImage(null), []);

  const pick = useCallback((file: File | undefined) => {
    if (!file) return;
    if (!ACCEPTED.includes(file.type)) {
      toast.error('Use a PNG, JPEG or WebP image');
      return;
    }
    setImage({ file, preview: URL.createObjectURL(file) });
  }, []);

  // One owner for the object URL: it is released whenever the image changes or goes away.
  useEffect(() => {
    if (!image) return;
    return () => URL.revokeObjectURL(image.preview);
  }, [image]);

  const onInputChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      pick(event.target.files?.[0]);
      event.target.value = '';
    },
    [pick],
  );

  const openPicker = useCallback(() => inputRef.current?.click(), []);

  return { image, inputRef, pick, clear, onInputChange, openPicker };
}

export type QuerySearchImageControl = ReturnType<typeof useQuerySearchImage>;
