"use client";

import {
  QUERY_ACCEPTED_IMAGE_MIME,
  QUERY_MAX_IMAGES,
  type AttachedImage,
} from "@/lib/query/query-interface-types";
import { useCallback, useRef, useState } from "react";
import { toast } from "sonner";

export function useQueryImages() {
  const [attachedImages, setAttachedImages] = useState<AttachedImage[]>([]);
  const imageInputRef = useRef<HTMLInputElement>(null);

  const addImages = useCallback(async (files: FileList | File[]) => {
    const candidates = Array.from(files).filter((file) =>
      QUERY_ACCEPTED_IMAGE_MIME.includes(
        file.type as (typeof QUERY_ACCEPTED_IMAGE_MIME)[number],
      ),
    );
    if (candidates.length === 0) return;

    const remaining = QUERY_MAX_IMAGES - attachedImages.length;
    if (remaining <= 0) {
      toast.error(`Maximum ${QUERY_MAX_IMAGES} images allowed`);
      return;
    }

    const toAdd = candidates.slice(0, remaining);
    const results = await Promise.all(
      toAdd.map(
        (file) =>
          new Promise<AttachedImage>((resolve, reject) => {
            const reader = new FileReader();
            reader.onload = () => {
              const dataUrl = reader.result as string;
              const base64 = dataUrl.split(",")[1] ?? "";
              resolve({ data: base64, mime_type: file.type, preview: dataUrl });
            };
            reader.onerror = reject;
            reader.readAsDataURL(file);
          }),
      ),
    );
    setAttachedImages((prev) => [...prev, ...results]);
  }, [attachedImages.length]);

  const removeImage = useCallback((idx: number) => {
    setAttachedImages((prev) => prev.filter((_, index) => index !== idx));
  }, []);

  const clearImages = useCallback(() => {
    setAttachedImages([]);
  }, []);

  const handleImageInputChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      if (event.target.files) {
        void addImages(event.target.files);
        event.target.value = "";
      }
    },
    [addImages],
  );

  const handlePaste = useCallback(
    (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      const items = Array.from(event.clipboardData.items).filter(
        (item) => item.kind === "file",
      );
      if (items.length > 0) {
        const files = items.map((item) => item.getAsFile()).filter(Boolean) as File[];
        if (files.length > 0) void addImages(files);
      }
    },
    [addImages],
  );

  const handleDrop = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      if (event.dataTransfer.files.length > 0) {
        void addImages(event.dataTransfer.files);
      }
    },
    [addImages],
  );

  const handleDragOver = useCallback((event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
  }, []);

  const getPayload = useCallback(() => {
    if (attachedImages.length === 0) return undefined;
    return attachedImages.map(({ data, mime_type }) => ({ data, mime_type }));
  }, [attachedImages]);

  return {
    attachedImages,
    imageInputRef,
    maxImages: QUERY_MAX_IMAGES,
    addImages,
    removeImage,
    clearImages,
    getPayload,
    handleImageInputChange,
    handlePaste,
    handleDrop,
    handleDragOver,
  };
}
