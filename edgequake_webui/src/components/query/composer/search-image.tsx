"use client";

import { Button } from "@/components/ui/button";
import { SEARCH_IMAGE_MIME, type QuerySearchImageControl } from "@/hooks/use-query-search-image";
import { ImageUp, X } from "lucide-react";
/* eslint-disable @next/next/no-img-element -- a blob: preview cannot use next/image */
import { useTranslation } from "react-i18next";

/** The hidden file input behind the "Search by image" button. */
// WHY the destructure: `react-hooks/refs` flags reading a ref object out of a props bag as a
// render-time ref access, because it cannot tell a hook's own ref from one it must not touch.
// Reading the two fields here makes it explicit that `ref={inputRef}` is the documented
// assignment; the `.current` read that the rule protects against happens only in `openPicker`.
export function SearchImageInput({ control }: { control: QuerySearchImageControl }) {
  const { t } = useTranslation();
  const { inputRef, onInputChange } = control;
  return (
    <input
      ref={inputRef}
      type="file"
      accept={SEARCH_IMAGE_MIME.join(",")}
      className="sr-only"
      aria-label={t("query.searchByImage.label", "Search by image")}
      onChange={onInputChange}
      data-testid="query-search-image-input"
    />
  );
}

/** Toolbar button. It is not rendered at all on a workspace that cannot embed images. */
export function SearchImageButton({ control }: { control: QuerySearchImageControl }) {
  const { t } = useTranslation();
  const label = t("query.searchByImage.label", "Search by image");
  return (
    <Button
      type="button"
      size="sm"
      variant="ghost"
      onClick={control.openPicker}
      className="h-8 gap-1.5 px-2 text-xs font-medium text-muted-foreground hover:text-foreground"
      aria-label={label}
      title={label}
      data-testid="query-search-image-button"
    >
      <ImageUp className="h-4 w-4" aria-hidden="true" />
      <span className="hidden sm:inline">{label}</span>
    </Button>
  );
}

/** The chosen image, with a way to take it back. */
export function SearchImageChip({ control }: { control: QuerySearchImageControl }) {
  const { t } = useTranslation();
  if (!control.image) return null;
  const { file, preview } = control.image;
  return (
    <div className="flex px-3 pt-3" data-testid="query-search-image-chip">
      <span className="group inline-flex h-9 max-w-[18rem] items-center gap-2 rounded-full bg-primary/[0.07] pl-1 pr-1.5 text-xs font-medium ring-1 ring-primary/15">
        <img
          src={preview}
          alt=""
          className="h-7 w-7 shrink-0 rounded-full object-cover"
          data-testid="query-search-image-thumb"
        />
        <span className="truncate" title={file.name}>
          {file.name}
        </span>
        <button
          type="button"
          onClick={control.clear}
          className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
          aria-label={t("query.searchByImage.remove", "Remove the search image")}
          data-testid="query-search-image-remove"
        >
          <X className="h-3 w-3" aria-hidden />
        </button>
      </span>
    </div>
  );
}
