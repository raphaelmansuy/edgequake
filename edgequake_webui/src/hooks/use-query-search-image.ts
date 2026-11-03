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

export interface QueryMessages {
  /** The user's own words. Empty when the whole request is a picture. */
  message: string;
  /** Text to show as the user's turn when there are no words of their own. */
  display: string;
}

/**
 * The (message, display) pair for one submission.
 *
 * WHY the model's `message` stays empty for a picture-only search: `query_image` is its own
 * retrieval mode. On a normal workspace the server embeds `message` as a *text* query vector
 * and fuses it with the image vector under equal weight (SPEC-180 09). Any invented sentence —
 * even a neutral "Find matches for this image" — is embedded as real text and competes with
 * the pixels. Measured on the SPEC-182 corpus: with that sentence the pizza probe returns a
 * beagle first and the pizza fourth; with an empty message the pizza is first.
 *
 * So the label a person reads ("Find matches for this image") is a *display* concern only.
 * Sending it as the query would be letting the UI choose the answer.
 */
export function searchImageMessages(
  text: string,
  hasImage: boolean,
  fallback: string,
): QueryMessages {
  const message = text.trim();
  if (message) return { message, display: message };
  if (hasImage) return { message: "", display: fallback };
  return { message: "", display: "" };
}

export type QuerySearchImageControl = ReturnType<typeof useQuerySearchImage>;
